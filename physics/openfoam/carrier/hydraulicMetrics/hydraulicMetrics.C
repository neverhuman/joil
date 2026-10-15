// Integral boundary mechanical-energy flux, in a horizontal zero-gravity case.
#include "argList.H"
#include "Time.H"
#include "fvMesh.H"
#include "volFields.H"
#include "surfaceFields.H"
#include "timeSelector.H"
#include "faceSet.H"
#include "surfaceInterpolate.H"
#include <fstream>
#include <iomanip>
using namespace Foam;

int main(int argc, char *argv[])
{
    Foam::timeSelector::addOptions();
    Foam::argList::addBoolOption("devicePlane", "integrate exact original outlet internal faceSet");
    #include "setRootCase.H"
    #include "createTime.H"
    Foam::instantList times(Foam::timeSelector::select0(runTime, args));
    if (times.empty()) FatalErrorInFunction << "no selected time" << Foam::exit(Foam::FatalError);
    runTime.setTime(times.last(), times.size()-1);
    #include "createMesh.H"
    Foam::volScalarField p(Foam::IOobject("p",runTime.name(),mesh,Foam::IOobject::MUST_READ,Foam::IOobject::NO_WRITE),mesh);
    Foam::volVectorField U(Foam::IOobject("U",runTime.name(),mesh,Foam::IOobject::MUST_READ,Foam::IOobject::NO_WRITE),mesh);
    Foam::surfaceScalarField phi(Foam::IOobject("phi",runTime.name(),mesh,Foam::IOobject::MUST_READ,Foam::IOobject::NO_WRITE),mesh);
    if (p.dimensions() != Foam::dimVelocity*Foam::dimVelocity || phi.dimensions() != Foam::dimVolume/Foam::dimTime)
        FatalErrorInFunction << "expected kinematic p and volume phi" << Foam::exit(Foam::FatalError);
    const double rho=998.;
    double power=0, qsum=0, qi=0, qu=0, qo=0, overflowPower=0;
    std::ofstream out("hydraulic-integrals.json");
    out << std::setprecision(15) << "{\n\"kind\":\"executed_boundary_mechanical_flux\",\n\"time\":" << runTime.value()
        << ",\n\"rho_kg_m3\":" << rho << ",\n\"gravity_m_s2\":[0,0,0],\n\"patches\":{\n";
    bool first=true;
    forAll(mesh.boundary(),i)
    {
        if (mesh.boundary()[i].size()==0) continue;
        const auto& name=mesh.boundary()[i].name();
        const auto& q=phi.boundaryField()[i];
        const auto& pb=p.boundaryField()[i];
        const auto& ub=U.boundaryField()[i];
        const auto& a=mesh.magSf().boundaryField()[i];
        const double flow=Foam::sum(q);
        const double area=Foam::sum(a);
        const double pp=Foam::sum(a*pb)/area*rho;
        const double pe=Foam::sum(q*(pb+.5*Foam::magSqr(ub)))*rho;
        double reverse=0;
        forAll(q,j) if ((name=="overflow" || name=="underflow") && q[j]<0) reverse-=q[j];
        power-=pe;qsum+=flow;
        if (name=="inlet1" || name=="inlet2") qi-=flow;
        if (name=="underflow") qu=flow;
        if (name=="overflow") {qo=flow;overflowPower=pe;}
        if(!first) out<<",\n";
        first=false;
        out << '"' << name << "\":{\"signed_flow_m3_s\":" << flow
            << ",\"area_m2\":" << area << ",\"area_average_static_pressure_pa\":" << pp
            << ",\"outward_mechanical_power_w\":" << pe << ",\"reverse_flow_m3_s\":" << reverse << '}';
    }
    if(qi<=0 || qu<=0) FatalErrorInFunction << "invalid feed/treated flow" << Foam::exit(Foam::FatalError);
    out << "}";
    if (args.optionFound("devicePlane"))
    {
        const Foam::faceSet plane(mesh,"deviceOverflow");
        const Foam::surfaceScalarField pf(Foam::fvc::interpolate(p));
        const Foam::surfaceVectorField uf(Foam::fvc::interpolate(U));
        double q=0,a=0,pa=0,pe=0,reverse=0;
        forAllConstIter(Foam::faceSet,plane,it)
        {
            const Foam::label face=it.key();
            if(face>=mesh.nInternalFaces()) FatalErrorInFunction << "device plane must be internal" << Foam::exit(Foam::FatalError);
            const auto sf=mesh.Sf()[face];
            const double sign=sf.z()<0?1:-1;
            if(std::abs(sf.z())/Foam::mag(sf)<.95) FatalErrorInFunction << "invalid original overflow normal" << Foam::exit(Foam::FatalError);
            const double flux=sign*phi[face];
            q+=flux;a+=Foam::mag(sf);pa+=Foam::mag(sf)*pf[face]*rho;
            pe+=flux*(pf[face]+.5*Foam::magSqr(uf[face]))*rho;
            if(flux<0) reverse-=flux;
        }
        if(a<=0) FatalErrorInFunction << "empty original device outlet" << Foam::exit(Foam::FatalError);
        out << ",\n\"original_device_overflow\":{\"faces\":" << plane.size()
            << ",\"area_m2\":" << a << ",\"signed_flow_m3_s\":" << q
            << ",\"reverse_flow_m3_s\":" << reverse << ",\"area_average_static_pressure_pa\":" << pa/a
            << ",\"outward_mechanical_power_w\":" << pe
            << ",\"device_hydraulic_loss_w\":" << power+overflowPower-pe
            << ",\"device_hydraulic_loss_j_per_treated_m3\":" << (power+overflowPower-pe)/qu
            << ",\"interpolation\":\"linear face p,U; native conservative phi\"}";
    }
    out << ",\n\"feed_m3_h\":" << qi*3600 << ",\n\"treated_m3_h\":" << qu*3600
        << ",\n\"reject_fraction\":" << qo/qi << ",\n\"water_closure_fraction\":" << std::abs(qsum)/qi
        << ",\n\"hydraulic_loss_w\":" << power << ",\n\"hydraulic_loss_j_per_treated_m3\":" << power/qu
        << ",\n\"limitations\":[\"single saved initializer field\",\"no pump or external manifold\",\"absolute pressure unknown\"]\n}\n";
    return 0;
}
