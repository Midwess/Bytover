//
//  DownloadUrl.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation

struct PublicUrlShareView: View {
    @EnvironmentObject var core: Core
    @State var password: String = ""
    @State var isObfuscated: Bool = true
    
    var body: some View {
        VStack(spacing: SpaceTheme.item.value) {
            Text("Password-protected shareable URL.")
                .modifier(Label2())
                .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                .multilineTextAlignment(.center)
            
            HStack {
                if isObfuscated {
                    SecureField("Enter password (optional)", text: $password)
                        .frame(width: .infinity, height: 22)
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .onChange(of: password) { newValue in
                            if newValue.count > 20 {
                                password = String(newValue.prefix(20))
                            }
                        }
                } else {
                    TextField("Enter password (optional)", text: $password)
                        .frame(width: .infinity, height: 22)
                        .modifier(Label1())
                        .foregroundColor(Theme.PrimaryText.color)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .onChange(of: password) { newValue in
                            if newValue.count > 20 {
                                password = String(newValue.prefix(20))
                            }
                        }
                }
                
                if !password.isEmpty {
                    Button(action: {
                        password = ""
                    }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                    }
                }
                
                Button(action: {
                    isObfuscated.toggle()
                }) {
                    Image(systemName: isObfuscated ? "eye.slash" : "eye")
                        .foregroundColor(Theme.PrimaryText.color.opacity(0.6))
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())
            
            Spacer()
            
            Button(action: {
                
            }) {
                Text("Upload")
                    .modifier(Label1())
                    .foregroundColor(Theme.PrimaryText.color)
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .background(Theme.BluePrimary.color)
            .clipShape(Capsule())
        }
    }
}

extension View {
    func placeholder<Content: View>(
        when shouldShow: Bool,
        alignment: Alignment = .leading,
        @ViewBuilder placeholder: () -> Content) -> some View {

        ZStack(alignment: alignment) {
            placeholder().opacity(shouldShow ? 1 : 0)
            self
        }
    }
}

#Preview {
    PublicUrlShareView()
        .environmentObject(CoreMock.withSelectedFileTransfers())
}
